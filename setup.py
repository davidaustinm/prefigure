from setuptools import setup

setup(
    name='prefigure',
    version='0.0.1',    
    description='PreFigure is a Python package for authoring mathematical diagrams',
    url='https://github.com/davidaustinm/prefigure',
    author='David Austin',
    author_email='davidaustinm@gmail.com',
    license='GPL',
    packages=['prefigure'],
    install_requires=['lxml==5.2.2',
                      'networkx==3.1',
                      'pycairo==1.26.0',
                      'scipy==1.10.1',
                      'numpy',                     
                      ],
    entry_points={
        'console_scripts': [
            'prefig = prefigure.parse:main',
        ],
    },
    classifiers=[
        'Operating System :: POSIX :: Linux',        
        'Programming Language :: Python :: 3.11',
    ],
)
